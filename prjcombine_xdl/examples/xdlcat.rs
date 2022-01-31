use prjcombine_xdl::Design;
use std::error::Error;
use std::fs;
use std::io;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "xdlcat", about = "Parse and reemit XDL.")]
struct Opt {
    src: String,
    dst: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let data = fs::read_to_string(opt.src)?;
    let design = Design::parse(&data)?;
    match opt.dst {
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
