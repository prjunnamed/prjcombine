use prjcombine::xilinx::vivado::parts::get_parts;
use prjcombine::toolchain::Toolchain;
use std::io;
use std::env;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    let tc = Toolchain::from_file(&args[1])?;
    let parts = get_parts(&tc)?;
    for part in parts {
        println!("{:?}", part);
    }
    Ok(())
}
