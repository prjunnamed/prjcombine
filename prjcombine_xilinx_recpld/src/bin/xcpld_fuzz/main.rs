mod backend;
mod bitstream;
mod collect;
mod fuzzers;

use std::{error::Error, path::PathBuf};

use backend::reverse_cpld;
use bitstream::reverse_bitstream;
use clap::Parser;
use prjcombine_toolchain::Toolchain;
use prjcombine_xilinx_recpld::{
    db::Database,
    hprep6::run_hprep6,
    vm6::{insert_dummy_obuf, prep_vm6},
};

#[derive(Parser)]
struct Args {
    tc: PathBuf,
    db: PathBuf,
    device: Option<String>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let tc = Toolchain::from_file(args.tc)?;
    let db = Database::from_file(args.db)?;
    for part in &db.parts {
        if let Some(ref filter) = args.device {
            if *filter != part.dev_name
                && *filter != format!("{d}{p}", d = part.dev_name, p = part.pkg_name)
            {
                continue;
            }
        }
        let device = &db.devices[part.device];
        let package = &db.packages[part.package];
        println!("DEV {d} {p}", d = part.dev_name, p = part.pkg_name);
        let bits = reverse_cpld(&tc, part, device, package, args.debug);
        bits.print(&mut std::io::stdout())?;
        let known_bits = bits.known_bits();
        let mut vm6 = prep_vm6(part, &device.device, package, &part.speeds[0]);
        insert_dummy_obuf(&mut vm6);
        let jed = run_hprep6(&tc, &vm6, None)?;
        let v2p = reverse_bitstream(
            &tc,
            device.device.kind,
            &part.dev_name,
            &part.pkg_name,
            jed.len(),
        );
        println!("BS RE DONE {}", v2p.main.len());
        for (i, b) in jed.iter().enumerate() {
            let (r, c) = v2p.main[i];
            if let Some(s) = known_bits.get(&i) {
                println!("BIT L{i} ({r}, {c}): {s}");
            } else {
                println!("BIT L{i} ({r}, {c}): UNK{v}", v = if *b { 1 } else { 0 });
            }
        }
        if let Some(uc) = v2p.usercode {
            for (i, (r, c)) in uc.into_iter().enumerate() {
                println!("    USERCODE {i} -> ({r}, {c})");
            }
        }
        if let Some(uc) = v2p.ues {
            for (i, (r, c)) in uc.into_iter().enumerate() {
                println!("    UES {i} -> ({r}, {c})");
            }
        }
        if let Some((r, c)) = v2p.done {
            println!("    DONE -> ({r}, {c})");
        }

        for (r, c) in v2p.rprot {
            println!("    READ PROT -> ({r}, {c})");
        }
        for (r, c) in v2p.wprot {
            println!("    WRITE PROT -> ({r}, {c})");
        }

        if let Some((c, r, a)) = v2p.dims {
            println!("    DIMS {c}×{r}, {a}");
        }
        for x in v2p.transfer {
            println!("    TRANSFER {x}");
        }
    }

    Ok(())
}
