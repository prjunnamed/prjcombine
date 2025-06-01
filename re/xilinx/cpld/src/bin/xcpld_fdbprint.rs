use std::{error::Error, path::PathBuf};

use clap::Parser;
use prjcombine_re_xilinx_cpld::fuzzdb::FuzzDb;

#[derive(Parser)]
struct Args {
    fdb: PathBuf,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = FuzzDb::from_file(args.fdb)?;
    for part in db.parts {
        println!("DEV {d} {p}", d = part.dev_name, p = part.pkg_name);
        part.bits.print(&mut std::io::stdout())?;
        let known_bits = part.bits.known_bits();
        println!("BS LEN {}", part.map.main.len());
        for (i, b) in part.blank.iter().enumerate() {
            let (r, c) = part.map.main[i];
            if let Some(s) = known_bits.get(&i) {
                println!("BIT L{i} ({r}, {c}): {s}");
            } else {
                println!("BIT L{i} ({r}, {c}): UNK{v}", v = if b { 1 } else { 0 });
            }
        }
        if let Some(uc) = part.map.usercode {
            for (i, (r, c)) in uc.into_iter().enumerate() {
                println!("    USERCODE {i} -> ({r}, {c})");
            }
        }
        if let Some(uc) = part.map.ues {
            for (i, (r, c)) in uc.into_iter().enumerate() {
                println!("    UES {i} -> ({r}, {c})");
            }
        }
        if let Some((r, c)) = part.map.done {
            println!("    DONE -> ({r}, {c})");
        }

        for (r, c) in part.map.rprot {
            println!("    READ PROT -> ({r}, {c})");
        }
        for (r, c) in part.map.wprot {
            println!("    WRITE PROT -> ({r}, {c})");
        }

        if let Some((c, r, a)) = part.map.dims {
            println!("    DIMS {c}×{r}, {a}");
        }
        for x in part.map.transfer {
            println!("    TRANSFER {x}");
        }
    }
    Ok(())
}
