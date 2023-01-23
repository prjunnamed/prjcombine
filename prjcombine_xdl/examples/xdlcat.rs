use clap::Parser;
use prjcombine_xdl::Design;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "xdlcat", about = "Parse and reemit XDL.")]
struct Args {
    src: PathBuf,
    dst: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let data = fs::read_to_string(args.src)?;
    let design = Design::parse(&data)?;
    match args.dst {
        None => {
            design.write(&mut io::stdout())?;
        }
        Some(fname) => {
            let mut f = fs::File::create(fname)?;
            design.write(&mut f)?;
        }
    }
    Ok(())
}
