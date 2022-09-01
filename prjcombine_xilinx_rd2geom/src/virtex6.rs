use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_rdverify::verify;
use prjcombine_virtex6_rd2db::{bond, grid, int};
use prjcombine_virtex6_rdverify::verify_bel;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(rd, &grid, &disabled, pins);
        bonds.push((pkg.clone(), Bond::Virtex6(bond)));
    }
    let eint = grid.expand_grid(&int_db, &disabled);
    verify(rd, &eint, |vrf, bel| verify_bel(&grid, vrf, bel));
    let disabled = disabled.into_iter().map(DisabledPart::Virtex6).collect();
    (
        make_device(rd, Grid::Virtex6(grid), bonds, disabled),
        Some(int_db),
    )
}
