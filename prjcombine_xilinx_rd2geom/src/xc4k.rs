use prjcombine_xilinx_geom::Grid;
use prjcombine_xilinx_geom::int::IntDb;
use prjcombine_xilinx_rawdump::Part;
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};

mod grid;
mod bond;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((pkg.clone(), bond::make_bond(&grid, pins)));
    }
    (
        make_device(rd, Grid::Xc4k(grid), bonds, BTreeSet::new()),
        None,
    )
}
