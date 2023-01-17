use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, Grid};
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};
use prjcombine_xc4k_rd2db::{bond, grid, int};
use prjcombine_xc4k_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let edev = grid.expand_grid(&int_db);
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&grid, pins);
        bonds.push((pkg.clone(), Bond::Xc4k(bond)));
    }
    if verify {
        verify_device(&edev, rd);
    }
    (
        make_device(rd, Grid::Xc4k(grid), bonds, BTreeSet::new()),
        Some(int_db),
    )
}
