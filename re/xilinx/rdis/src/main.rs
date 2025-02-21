use std::{error::Error, path::PathBuf};

use arrayref::array_ref;
use clap::Parser;
use prjcombine_re_xilinx_geom::GeomDb;
use prjcombine_xilinx_bitstream::{KeyData, Reg};

#[derive(Debug, Parser)]
#[command(name = "xrdis", about = "Disasm xilinx bitstream.")]
struct Args {
    geomdb: PathBuf,
    bitfile: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = GeomDb::from_file(args.geomdb)?;
    let mut bitdata = std::fs::read(args.bitfile)?;
    assert_eq!(
        bitdata[..13],
        [
            0x00, 0x09, 0x0f, 0xf0, 0x0f, 0xf0, 0x0f, 0xf0, 0x0f, 0xf0, 0x00, 0x00, 0x01
        ]
    );
    let mut pos = 13;
    let mut meta = vec![];
    for l in [b'a', b'b', b'c', b'd'] {
        assert_eq!(bitdata[pos], l);
        pos += 1;
        let len = u16::from_be_bytes(*array_ref!(bitdata, pos, 2)) as usize;
        pos += 2;
        meta.push(String::from_utf8(bitdata[pos..pos + len - 1].to_vec()).unwrap());
        pos += len;
    }
    assert_eq!(bitdata[pos], b'e');
    pos += 1;
    let len = u32::from_be_bytes(*array_ref!(bitdata, pos, 4)) as usize;
    pos += 4;
    assert_eq!(pos + len, bitdata.len());
    bitdata.drain(0..pos);
    let part = format!("xc{}", meta[1]);
    let (device, _bond) = 'a: {
        for device in &db.devices {
            for bond in device.bonds.values() {
                let curpart = format!("{}{}", device.name, bond.name);
                if part == curpart {
                    break 'a (device, bond);
                }
            }
        }
        panic!("umm unknown device {part}?");
    };
    let gedev = db.expand_grid(device);
    let bs_geom = gedev.bs_geom();
    let bitstream = prjcombine_xilinx_bitstream::parse(bs_geom, &bitdata, &KeyData::None);
    for (die, dbs) in &bitstream.die {
        if let Some(val) = dbs.regs[Reg::Idcode] {
            println!("DIE {die} IDCODE {val:08x}");
        }
    }
    Ok(())
}
