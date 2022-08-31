use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Grid, Bond};
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};
use prjcombine_xc4k_rd2db::{bond, grid};

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&grid, pins);
        bonds.push((pkg.clone(), Bond::Xc4k(bond)));
    }
    (
        make_device(rd, Grid::Xc4k(grid), bonds, BTreeSet::new()),
        None,
    )
}
