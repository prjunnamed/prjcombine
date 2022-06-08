use std::collections::{BTreeMap, BTreeSet, HashMap};
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use prjcombine_xilinx_geom::{self as geom, Bond, BondPin};
use prjcombine_xilinx_geom::xc5200;

use crate::grid::{extract_int, PreDevice, make_device};

fn make_grid(rd: &Part) -> xc5200::Grid {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &[
        "CENTER",
        "LL",
        "LR",
        "UL",
        "UR",
    ], &[]);
    xc5200::Grid {
        columns: int.cols.len() as u32,
        rows: int.rows.len() as u32,
    }
}

fn make_bond(grid: &xc5200::Grid, pins: &[PkgPin]) -> Bond {
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

pub fn ingest(rd: &Part) -> PreDevice {
    let grid = make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(&grid, pins),
        ));
    }
    make_device(rd, geom::Grid::Xc5200(grid), bonds, BTreeSet::new())
}
