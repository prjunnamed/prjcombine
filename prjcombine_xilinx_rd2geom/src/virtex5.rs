use prjcombine_xilinx_geom::{int::IntDb, Grid};
use prjcombine_xilinx_rawdump::Part;
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};
use crate::verify::Verifier;

mod bond;
mod grid;
mod int;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((pkg.clone(), bond::make_bond(&grid, pins)));
    }
    let eint = grid.expand_grid(&int_db);
    let vrf = Verifier::new(rd, &eint);
    vrf.finish();
    (
        make_device(rd, Grid::Virtex5(grid), bonds, BTreeSet::new()),
        Some(int_db),
    )
}
