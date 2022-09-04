use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, Grid};
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};
use prjcombine_rdverify::verify;
use prjcombine_virtex4_rd2db::{bond, grid, int};
use prjcombine_virtex4_rdverify::{verify_bel, verify_extra};

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&grid, pins);
        bonds.push((pkg.clone(), Bond::Virtex4(bond)));
    }
    let eint = grid.expand_grid(&int_db);
    verify(
        rd,
        &eint,
        |vrf, ctx| verify_bel(&grid, vrf, ctx),
        |vrf| verify_extra(&grid, vrf),
    );
    (
        make_device(rd, Grid::Virtex4(grid), bonds, BTreeSet::new()),
        Some(int_db),
    )
}
