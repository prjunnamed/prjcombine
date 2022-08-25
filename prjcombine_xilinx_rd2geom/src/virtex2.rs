use std::collections::BTreeSet;

use prjcombine_xilinx_rawdump::Part;
use prjcombine_xilinx_geom::{Grid, int::IntDb};

use crate::grid::{PreDevice, make_device};
use crate::verify::verify;

mod verify;
mod int_v2;
mod int_s3;
mod grid;
mod bond;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let int_db = if rd.family.starts_with("virtex2") {
        int_v2::make_int_db(rd)
    } else {
        int_s3::make_int_db(rd)
    };
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            bond::make_bond(&grid, pins),
        ));
    }
    let eint = grid.expand_grid(&int_db);

    verify(rd, &eint, |vrf, slr, node, bid| {
        verify::verify_bel(&grid, vrf, slr, node, bid)
    });
    (make_device(rd, Grid::Virtex2(grid), bonds, BTreeSet::new()), Some(int_db))
}
