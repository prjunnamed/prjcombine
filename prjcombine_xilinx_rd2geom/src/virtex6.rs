use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_virtex6::expand_grid;
use prjcombine_virtex6_rd2db::{bond, grid, int};
use prjcombine_virtex6_rdverify::verify;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let edev = expand_grid(&grid, &int_db, &disabled);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&edev, pins);
        bonds.push((pkg.clone(), Bond::Virtex6(bond)));
    }
    verify(&edev, rd);
    let disabled = disabled.into_iter().map(DisabledPart::Virtex4).collect();
    (
        make_device(rd, Grid::Virtex4(grid), bonds, disabled),
        Some(int_db),
    )
}
