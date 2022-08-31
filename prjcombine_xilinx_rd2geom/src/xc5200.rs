use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Grid, Bond};
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};
use prjcombine_rdverify::verify;
use prjcombine_xc5200_rd2db::{bond, grid, int};
use prjcombine_xc5200_rdverify::verify_bel;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let edev = grid.expand_grid(&int_db);
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&edev, pins);
        bonds.push((pkg.clone(), Bond::Xc5200(bond)));
    }
    verify(rd, &edev.egrid, |vrf, ctx| verify_bel(&edev, vrf, ctx));
    (
        make_device(rd, Grid::Xc5200(grid), bonds, BTreeSet::new()),
        Some(int_db),
    )
}
