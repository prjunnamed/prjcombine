use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{int::IntDb, Grid};

use crate::db::{make_device, PreDevice};
use crate::verify::verify;

mod bond;
mod grid;
mod int;
mod verify;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let edev = grid.expand_grid(&disabled, &int_db);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((pkg.clone(), bond::make_bond(&edev, pins)));
    }
    verify(rd, &edev.egrid, |vrf, ctx| {
        verify::verify_bel(&edev, vrf, ctx)
    });
    (
        make_device(rd, Grid::Virtex(grid), bonds, disabled),
        Some(int_db),
    )
}
