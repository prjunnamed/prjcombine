use prjcombine_toolchain::Toolchain;
use prjcombine_vivado_dump::parts::get_parts;
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let tc = Toolchain::from_file(&args[1])?;
    let parts = get_parts(&tc)?;
    for part in parts {
        println!("{part:?}");
    }
    Ok(())
}
