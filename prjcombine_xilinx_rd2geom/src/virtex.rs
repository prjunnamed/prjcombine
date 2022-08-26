use prjcombine_xilinx_geom::{int::IntDb, Grid};
use prjcombine_xilinx_rawdump::Part;

use crate::db::{make_device, PreDevice};
use crate::verify::verify;

mod bond;
mod grid;
mod int;
mod verify;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((pkg.clone(), bond::make_bond(&grid, pins)));
    }
    let eint = grid.expand_grid(&disabled, &int_db);
    verify(rd, &eint, |vrf, slr, node, bid| {
        verify::verify_bel(&grid, vrf, slr, node, bid)
    });
    (
        make_device(rd, Grid::Virtex(grid), bonds, disabled),
        Some(int_db),
    )
}
