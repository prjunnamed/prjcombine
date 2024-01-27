use std::{error::Error, path::PathBuf};

use clap::Parser;
use itertools::Itertools;
use prjcombine_types::IoId;
use prjcombine_xilinx_cpld::device::{DeviceKind, PkgPin};
use prjcombine_xilinx_cpld::types::ImuxInput;
use prjcombine_xilinx_recpld::db::Database;
use unnamed_entity::EntityId;

#[derive(Parser)]
struct Args {
    db: PathBuf,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = Database::from_file(args.db)?;
    for (did, device) in &db.devices {
        print!("DEVICE {did}:", did = did.to_idx());
        for part in &db.parts {
            if part.device == did {
                print!(" {dev}-{pkg}", dev = part.dev_name, pkg = part.pkg_name);
            }
        }
        println!();
        println!("\tKIND: {:?}", device.device.kind);
        println!("\tFBS: {}", device.device.fbs);
        println!("\tIPADS: {}", device.device.ipads);
        println!("\tBANKS: {}", device.device.banks);
        if device.device.kind == DeviceKind::Xpla3 {
            println!("\tFB GROUPS: {}", device.device.fb_groups);
            for (fb, fbg) in &device.device.fb_group {
                println!(
                    "\tFB{fb}: FB GROUP {fbg}",
                    fb = fb.to_idx(),
                    fbg = fbg.to_idx()
                );
            }
        }
        if device.device.kind == DeviceKind::Xc9500 {
            println!("\tHAS FBK: {:?}", device.device.has_fbk);
        }
        if device.device.kind == DeviceKind::Coolrunner2 {
            println!("\tHAS VREF: {:?}", device.device.has_vref);
        }
        for (&ioid, io) in device.device.io.iter().sorted_by_key(|a| a.0) {
            match ioid {
                IoId::Ipad(ip) => print!("\tIPAD{ip}:"),
                IoId::Mc((fb, mc)) => print!("\tIO{fb}_{mc}:"),
            }
            print!(
                " PAD{pad} BANK {bank}",
                pad = io.pad,
                bank = io.bank.to_idx()
            );
            if let Some((idx, _)) = device.device.clk_pads.iter().find(|(_, &y)| y == ioid) {
                print!(" FCLK{idx}", idx = idx.to_idx());
            }
            if let Some((idx, _)) = device.device.oe_pads.iter().find(|(_, &y)| y == ioid) {
                print!(" FOE{idx}", idx = idx.to_idx());
            }
            if device.device.sr_pad == Some(ioid) {
                print!(" FSR");
            }
            if device.device.dge_pad == Some(ioid) {
                print!(" DGE");
            }
            if device.device.cdr_pad == Some(ioid) {
                print!(" CDR");
            }
            if let Some(j) = io.jtag {
                print!(" {j:?}");
            }
            println!();
        }
        for (imid, inps) in &device.imux {
            print!("\tIMUX {imid}:", imid = imid.to_idx());
            for (inp, idx) in inps.iter().sorted() {
                let finp = match inp {
                    ImuxInput::Ibuf(IoId::Mc(mc)) => {
                        format!("MCIO{f}_{m}({idx})", f = mc.0.to_idx(), m = mc.1.to_idx())
                    }
                    ImuxInput::Ibuf(IoId::Ipad(ip)) => {
                        format!("IPAD{ip}({idx})", ip = ip.to_idx())
                    }
                    ImuxInput::Fbk(mc) => {
                        format!("FBK{m}({idx})", m = mc.to_idx())
                    }
                    ImuxInput::Mc(mc) => {
                        format!("MC{f}_{m}({idx})", f = mc.0.to_idx(), m = mc.1.to_idx())
                    }
                    ImuxInput::Pup => format!("PUP({idx})"),
                    ImuxInput::Uim => "UIM".to_string(),
                };
                print!(" {finp:13}");
            }
            println!();
        }
    }
    for (pid, pkg) in &db.packages {
        print!("PACKAGE {pid}:", pid = pid.to_idx());
        for part in &db.parts {
            if part.package == pid {
                print!(" {dev}-{pkg}", dev = part.dev_name, pkg = part.pkg_name);
            }
        }
        println!();
        for (&f, &t) in &pkg.spec_remap {
            print!("\tREMAP ");
            match f {
                IoId::Ipad(ip) => print!("IPAD{ip}", ip = ip.to_idx()),
                IoId::Mc(mc) => print!("IO{f}_{m}", f = mc.0.to_idx(), m = mc.1.to_idx()),
            }
            print!(" -> ");
            match t {
                IoId::Ipad(ip) => print!("IPAD{ip}", ip = ip.to_idx()),
                IoId::Mc(mc) => print!("IO{f}_{m}", f = mc.0.to_idx(), m = mc.1.to_idx()),
            }
            println!();
        }
        for (bid, &bank) in &pkg.banks {
            if let Some(bank) = bank {
                println!("\tBANK {bid}: {bank}", bid = bid.to_idx());
            } else {
                println!("\tBANK {bid}: [UNNAMED]", bid = bid.to_idx());
            }
        }
        for (pin, info) in pkg.pins.iter().sorted_by_key(|&(k, _)| k) {
            print!("\t{pin}: ");
            match info {
                PkgPin::Nc => print!("NC"),
                PkgPin::Gnd => print!("GND"),
                PkgPin::VccInt => print!("VCCINT"),
                PkgPin::VccIo(b) => print!("VCCIO{b}", b = b.to_idx()),
                PkgPin::VccAux => print!("VCCAUX"),
                PkgPin::Jtag(pin) => print!("{pin:?}"),
                PkgPin::PortEn => print!("PORT_EN"),
                PkgPin::Io(IoId::Mc(mc)) => {
                    print!("IO{f}_{m}", f = mc.0.to_idx(), m = mc.1.to_idx())
                }
                PkgPin::Io(IoId::Ipad(ip)) => print!("IPAD{ip}", ip = ip.to_idx()),
            }
            println!();
        }
    }
    for part in &db.parts {
        println!(
            "PART {dev}-{pkg} DEVICE {did} PACKAGE {pid} SPEEDS {speeds:?} NDS {nds_version} VM6 FAMILY {vm6_family} DEV {vm6_dev} DEVPKG {vm6_devpkg}",
            dev = part.dev_name,
            pkg = part.pkg_name,
            did = part.device.to_idx(),
            pid = part.package.to_idx(),
            speeds = part.speeds,
            nds_version = part.nds_version,
            vm6_family = part.vm6_family,
            vm6_dev = part.vm6_dev,
            vm6_devpkg = part.vm6_devpkg,
        );
    }
    Ok(())
}
