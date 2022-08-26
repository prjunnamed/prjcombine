use prjcombine_xilinx_geom::pkg::{Bond, BondPin};
use prjcombine_xilinx_geom::xc5200::Grid;
use prjcombine_xilinx_rawdump::PkgPin;
use std::collections::{BTreeMap, HashMap};

pub fn make_bond(grid: &Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                BondPin::IoByCoord(io)
            } else {
                println!("UNK PAD {}", pad);
                continue;
            }
        } else {
            println!("UNK FUNC {}", pin.func);
            continue;
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks: BTreeMap::new(),
    }
}
