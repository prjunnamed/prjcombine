use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::int::IntDb;
use prjcombine_xilinx_geom::Grid;
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};
use crate::verify::verify;

mod bond;
mod grid;
mod int;
mod verify;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let edev = grid.expand_grid(&int_db);
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((pkg.clone(), bond::make_bond(&edev, pins)));
    }
    verify(rd, &edev.egrid, |vrf, ctx| {
        verify::verify_bel(&edev, vrf, ctx)
    });
    (
        make_device(rd, Grid::Xc5200(grid), bonds, BTreeSet::new()),
        Some(int_db),
    )
}
